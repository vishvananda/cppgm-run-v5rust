struct C {};
typedef int word;
int value;
template<class T> struct P { typedef int word; };
template<int> struct Q {};

void syntax() {
  sizeof(P<P<C>>(1));
  typeid(P<P<C>>(1));
  alignof(P<P<C>>(1));
  (P<P<C>>(1));
  (P<P<C>>)value;
  sizeof(C __attribute__((vector_size(16))));
  typeid(P<C> const *);
  typeid(value).member;
  sizeof(P<C>(1)[0]);
  sizeof((value)[0]);
}

Q<(sizeof(P<(value > 1)>))> greater_operand;
Q<(noexcept(value > 1))> greater_noexcept;
Q<(sizeof((word)value))> cast_operand;
