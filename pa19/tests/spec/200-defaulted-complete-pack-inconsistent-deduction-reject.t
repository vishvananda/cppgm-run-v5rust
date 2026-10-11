template<class A=int,class B=int,class C=int>struct box{};
template<class T,class...Ts>int selected(const box<T,Ts...>&,const box<T,Ts...>&){return sizeof...(Ts);}
int main(){return selected(box<int,char>(),box<int,long>());}
