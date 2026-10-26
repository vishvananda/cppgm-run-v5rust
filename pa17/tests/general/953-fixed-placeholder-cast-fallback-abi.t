template<class T>struct X{template<class U>auto f(U u)->decltype(static_cast<decltype(T())>(u)){return u;}};int main(){X<int>x;return x.f<int>(1)-1;}
