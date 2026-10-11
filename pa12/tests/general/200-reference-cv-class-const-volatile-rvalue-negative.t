// C++11 reference initialization: N3485 8.5.3.
struct S{int x;};int main(){const volatile S& r=S{3};return r.x;}
