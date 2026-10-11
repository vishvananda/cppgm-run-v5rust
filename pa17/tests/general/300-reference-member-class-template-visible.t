template<class T> struct Outer { template<class U> struct Inner { U val; }; };
Outer<int>::Inner<float> *f();
