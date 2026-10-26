template<class T>struct X{int call(T t){return t;}template<class U>auto f(U u)->decltype(call(u)){return call(u);}};int main(){X<int>x;return x.f<int>(2)-2;}
