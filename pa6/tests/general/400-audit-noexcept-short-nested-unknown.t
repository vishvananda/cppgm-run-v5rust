// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
void f() noexcept(true || (1.0 + missing));
