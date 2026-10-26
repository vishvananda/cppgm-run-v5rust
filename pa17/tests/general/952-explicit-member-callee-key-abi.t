template<class T>struct X{template<class V>int call(V t){return t;}template<class U>auto f(U u)->decltype(call<U>(u)){return call<U>(u);}};int main(){X<int>x;return x.f<int>(2)-2;}
