// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
void f(int (*p)()) noexcept(p());
