package dev.lm.tester.config

/**
 * Global logging configuration.
 * Set verbose/debug to true to enable detailed logging.
 */
object LogConfig {
    @Volatile var verbose: Boolean = false
    @Volatile var debug: Boolean = false
    
    /**
     * Configure logging from environment or command line args
     */
    fun configure(verbose: Boolean = false, debug: Boolean = false) {
        this.verbose = verbose
        this.debug = debug
        if (verbose || debug) {
            println("[CONFIG] Logging: verbose=$verbose, debug=$debug")
        }
    }
}
