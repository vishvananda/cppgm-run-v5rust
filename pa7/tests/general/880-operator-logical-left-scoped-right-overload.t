// Scalar analysis failure ordering, compared with the frozen C++ owner.
enum class E{v};void f(int);void f(long);void g(E e){e&&f;}
