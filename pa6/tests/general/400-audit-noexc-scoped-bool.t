// PA6 checkpoint audit: frozen C++ declaration/exception behavior.
enum class E{a}; void f() noexcept(true || E::a);
