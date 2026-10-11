// A written suffix makes the whole list non-deduced; other arguments can supply the pack.
template<class A=int,class B=int,class C=int>struct box{};
template<class...Ts>int explicit_pack(box<Ts...,int>){return sizeof...(Ts);}
template<class...Ts>int separate_pack(box<Ts...,int>,Ts...){return sizeof...(Ts);}
int main(){return explicit_pack<char,long>(box<char,long,int>())==2 &&
                  separate_pack(box<char,long,int>(),char(1),long(2))==2 ? 0 : 1;}
