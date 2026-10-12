// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
int x; void f() noexcept(x+missing);
