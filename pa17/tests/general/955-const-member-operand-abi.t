template<class T>struct X{T member;template<class U>auto f(U u)const->decltype(member+u){return member+u;}};int main(){X<int>x;x.member=1;return x.f<int>(2)-3;}
