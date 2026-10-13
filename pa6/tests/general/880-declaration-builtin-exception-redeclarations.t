// Builtin declarations retain implicit exception facts across explicit redeclarations.
void __builtin_abort()noexcept;void __builtin_abort()noexcept(false);
