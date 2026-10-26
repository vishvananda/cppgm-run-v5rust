template<class T>struct X{T member;template<class U>auto f(U u)->decltype(X::member+u){return X::member+u;}};int main(){X<int>x;x.member=1;return x.f<int>(2)-3;}
