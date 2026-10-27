template<class T,class U> struct Pair{};
template<class...T> struct Outer {
 template<class...U> struct Inner {
  void f() throw(Pair<T,U>...) {}
 };
};
int main(){return sizeof(Outer<int,char>::Inner<short>)-1;}
