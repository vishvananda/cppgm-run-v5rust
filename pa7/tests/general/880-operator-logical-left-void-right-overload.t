// Scalar analysis failure ordering, compared with the frozen C++ owner.
void h();void f(int);void f(long);void g(){h()&&f;}
