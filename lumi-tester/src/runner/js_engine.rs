//! JavaScript evaluation engine using boa_engine
//!
//! Provides JavaScript expression evaluation for:
//! - evalScript command
//! - when conditions in runFlow
//! - assertTrue expressions

#[allow(unused_imports)]
use boa_engine::{
    native_function::NativeFunction, object::ObjectInitializer, property::Attribute, Context,
    JsResult, JsString, JsValue, Source,
};
use std::collections::HashMap;

/// JavaScript evaluation engine
pub struct JsEngine {
    context: Context,
}

impl JsEngine {
    /// Create a new JavaScript engine instance
    pub fn new() -> Self {
        Self {
            context: Context::default(),
        }
    }

    /// Set variables in the JavaScript context
    pub fn set_vars(&mut self, vars: &HashMap<String, String>) {
        for (key, value) in vars {
            // Try to parse as number, otherwise treat as string
            let js_code = if value.parse::<f64>().is_ok() {
                format!("var {} = {};", key, value)
            } else if value == "true" || value == "false" {
                format!("var {} = {};", key, value)
            } else {
                format!("var {} = \"{}\";", key, value.replace('"', "\\\""))
            };

            let _ = self.context.eval(Source::from_bytes(&js_code));
        }
    }

    /// Execute a script file content and return the 'output' global variable as a JSON string
    pub fn execute_script_with_output(&mut self, script_content: &str) -> Result<String, String> {
        // 1. Inject 'output' object
        let output_obj = ObjectInitializer::new(&mut self.context).build();
        self.context
            .register_global_property(
                JsString::from("output"),
                output_obj.clone(),
                Attribute::all(),
            )
            .map_err(|e| format!("JS Error: {}", e))?;

        // 3. Inject 'json' function (alias for JSON.parse)
        fn json_parse(
            _this: &JsValue,
            args: &[JsValue],
            context: &mut Context,
        ) -> JsResult<JsValue> {
            if let Some(_arg) = args.get(0) {
                // Access JSON.parse from global object
                let global = context.global_object();
                let json_obj = global.get(JsString::from("JSON"), context)?;
                let parse_fn = json_obj
                    .as_object()
                    .ok_or_else(|| {
                        boa_engine::JsError::from_opaque(JsValue::from(JsString::from(
                            "JSON is not an object",
                        )))
                    })?
                    .get(JsString::from("parse"), context)?;

                if let Some(func) = parse_fn.as_callable() {
                    return func.call(&JsValue::undefined(), args, context);
                }
            }
            Err(boa_engine::JsError::from_opaque(JsValue::from(
                JsString::from("json() requires a string"),
            )))
        }

        // Use FunctionObjectBuilder
        use boa_engine::object::FunctionObjectBuilder;
        let json_func = FunctionObjectBuilder::new(
            self.context.realm(),
            NativeFunction::from_fn_ptr(json_parse),
        )
        .length(1)
        .build();

        self.context
            .register_global_property(JsString::from("json"), json_func, Attribute::all())
            .map_err(|e| format!("JS Error: {}", e))?;

        // Execute the script
        self.context
            .eval(Source::from_bytes(script_content))
            .map_err(|e| format!("JS Execution Error: {}", e))?;

        // Extract 'output' object
        // Use JSON.stringify(output) to get the JSON string
        let global = self.context.global_object();
        let json_obj = global
            .get(JsString::from("JSON"), &mut self.context)
            .map_err(|e| format!("Failed to get JSON object: {}", e))?;

        let stringify_fn = json_obj
            .as_object()
            .ok_or("JSON is not an object".to_string())?
            .get(JsString::from("stringify"), &mut self.context)
            .map_err(|e| format!("Failed to get JSON.stringify: {}", e))?;

        let output_val = global
            .get(JsString::from("output"), &mut self.context)
            .map_err(|e| format!("Failed to get output var: {}", e))?;

        if let Some(func) = stringify_fn.as_callable() {
            let json_str_val = func
                .call(&JsValue::undefined(), &[output_val], &mut self.context)
                .map_err(|e| format!("Failed to stringify output: {}", e))?;

            if let Some(s) = json_str_val.as_string() {
                return Ok(s.to_std_string_escaped());
            }
        }

        Ok("{}".to_string())
    }

    /// Evaluate a JavaScript expression and return the result as a string
    pub fn eval(&mut self, expression: &str) -> Result<String, String> {
        match self.context.eval(Source::from_bytes(expression)) {
            Ok(result) => Ok(js_value_to_string(&result)),
            Err(e) => Err(format!("JavaScript error: {}", e)),
        }
    }

    /// Evaluate a JavaScript expression and return as boolean
    pub fn eval_bool(&mut self, expression: &str) -> Result<bool, String> {
        match self.context.eval(Source::from_bytes(expression)) {
            Ok(result) => Ok(result.to_boolean()),
            Err(e) => Err(format!("JavaScript error: {}", e)),
        }
    }

    /// Evaluate an assignment expression and return the assigned value
    pub fn eval_assignment(
        &mut self,
        expression: &str,
    ) -> Result<Option<(String, String)>, String> {
        // Check if it's an assignment (contains = but not == or != or <= or >=)
        if let Some(idx) = expression.find('=') {
            let before = &expression[..idx];
            let after = &expression[idx + 1..];

            // Skip if it's a comparison
            if !before.ends_with('!')
                && !before.ends_with('<')
                && !before.ends_with('>')
                && !after.starts_with('=')
            {
                // Evaluate the full expression
                match self.context.eval(Source::from_bytes(expression)) {
                    Ok(result) => {
                        let var_name = before.trim().to_string();
                        let value = js_value_to_string(&result);
                        Ok(Some((var_name, value)))
                    }
                    Err(e) => Err(format!("JavaScript error: {}", e)),
                }
            } else {
                Ok(None)
            }
        } else {
            // Not an assignment, just evaluate
            self.eval(expression)?;
            Ok(None)
        }
    }
}

impl Default for JsEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert JsValue to String representation
fn js_value_to_string(value: &JsValue) -> String {
    if value.is_undefined() {
        "undefined".to_string()
    } else if value.is_null() {
        "null".to_string()
    } else if let Some(b) = value.as_boolean() {
        b.to_string()
    } else if let Some(n) = value.as_number() {
        if n.fract() == 0.0 {
            (n as i64).to_string()
        } else {
            n.to_string()
        }
    } else if let Some(s) = value.as_string() {
        s.to_std_string_escaped()
    } else {
        format!("{:?}", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_arithmetic() {
        let mut engine = JsEngine::new();
        assert_eq!(engine.eval("1 + 2").unwrap(), "3");
        assert_eq!(engine.eval("10 - 3").unwrap(), "7");
        assert_eq!(engine.eval("4 * 5").unwrap(), "20");
        assert_eq!(engine.eval("15 / 3").unwrap(), "5");
    }

    #[test]
    fn test_eval_boolean() {
        let mut engine = JsEngine::new();
        assert!(engine.eval_bool("true").unwrap());
        assert!(!engine.eval_bool("false").unwrap());
        assert!(engine.eval_bool("5 > 3").unwrap());
        assert!(!engine.eval_bool("2 > 5").unwrap());
    }

    #[test]
    fn test_eval_with_vars() {
        let mut engine = JsEngine::new();
        let mut vars = HashMap::new();
        vars.insert("count".to_string(), "5".to_string());
        vars.insert("name".to_string(), "test".to_string());

        engine.set_vars(&vars);

        assert_eq!(engine.eval("count + 1").unwrap(), "6");
        assert_eq!(engine.eval("name").unwrap(), "test");
    }

    #[test]
    fn test_assignment() {
        let mut engine = JsEngine::new();
        let result = engine.eval_assignment("x = 10 + 5").unwrap();
        assert_eq!(result, Some(("x".to_string(), "15".to_string())));

        // Verify variable is set
        assert_eq!(engine.eval("x").unwrap(), "15");
    }
}
