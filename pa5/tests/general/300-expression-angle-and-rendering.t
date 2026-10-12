int a,b,c,d;
struct X {};
template<int N> struct P {};
P<::new int(a>b)> allocation;
P<::delete[] a> deallocation;
P<sizeof(((a)))> extent;
P<(a?b>c:d)> condition;
P<static_cast<int>(a)> cast_value;
void f(){ call(alignof(const X()), a[{b,c}], unsigned long(1)); }
