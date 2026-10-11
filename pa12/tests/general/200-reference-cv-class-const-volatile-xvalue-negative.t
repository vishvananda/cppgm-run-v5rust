// C++11 reference initialization: N3485 8.5.3.
struct S{int x;};int main(){S s{3};const volatile S& r=static_cast<S&&>(s);return r.x;}
