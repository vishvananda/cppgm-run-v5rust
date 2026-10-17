// Different bound packs in one base pattern cannot expand in lockstep. expect reject
template<class T, class U> struct Base {};
template<class... T> struct Outer {
  template<class... U> struct Inner : Base<T, U>... {};
};
Outer<int, long>::Inner<char> value;
