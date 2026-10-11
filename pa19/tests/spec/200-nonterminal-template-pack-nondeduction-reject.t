template<class A=int,class B=int,class C=int>struct box{};
template<class T,class...Ts>void selected(box<T,Ts...,int>){}
int main(){selected(box<char,long,int>());}
