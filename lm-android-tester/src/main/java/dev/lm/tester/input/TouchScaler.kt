package dev.lm.tester.input

object TouchScaler {
    @Volatile var scaleX: Float = 1.0f
    @Volatile var scaleY: Float = 1.0f
    
    fun configure(deviceWidth: Int, deviceHeight: Int, encoderWidth: Int, encoderHeight: Int) {
        scaleX = deviceWidth.toFloat() / encoderWidth.toFloat()
        scaleY = deviceHeight.toFloat() / encoderHeight.toFloat()
    }
    
    fun transform(x: Float, y: Float): Pair<Float, Float> {
        return Pair(x * scaleX, y * scaleY)
    }
}
