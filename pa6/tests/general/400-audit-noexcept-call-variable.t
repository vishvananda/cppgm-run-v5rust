// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
int (*p)(); void f() noexcept(p());
