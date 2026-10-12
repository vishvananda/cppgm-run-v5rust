// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
enum class E{a}; void f() noexcept(E::a==0);
