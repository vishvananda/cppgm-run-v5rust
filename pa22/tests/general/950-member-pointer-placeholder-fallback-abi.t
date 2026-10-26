template<class T>struct X{template<class U>int f(decltype(T()) U::*p){return 0;}};struct Y{int value;};int main(){X<int>x;return x.f<Y>(&Y::value);}
