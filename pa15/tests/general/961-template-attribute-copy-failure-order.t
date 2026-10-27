template<class T,class U> struct Pair {};
template<class...> struct Align {};
template<class...T> struct Outer {
  template<class...U> struct Inner {
    alignas(Align<Pair<T,U>...>)
    alignas(Align<Pair<T,U>...>) int member;
  };
};
int main(){return sizeof(Outer<int,char>::Inner<short>);}
