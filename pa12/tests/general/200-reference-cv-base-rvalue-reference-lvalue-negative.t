// C++11 reference initialization: N3485 8.5.3.
struct B{int x;};struct D:B{};int main(){D d;d.x=3;B&& r=d;return r.x;}
