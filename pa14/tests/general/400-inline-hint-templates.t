// lowir.md inline_hint: a template specialization is an inline function when
// the template is declared inline or (a member) defined in its class template;
// a member defined outside its class template without inline, and a function
// template not declared inline, are not.
template<class T> struct Box
{
  T value;
  T in_class() const { return value; }
  T out_of_class() const;
  inline T inline_out_of_class() const;
};
template<class T> T Box<T>::out_of_class() const { return value + 1; }
template<class T> T Box<T>::inline_out_of_class() const { return value + 2; }
template<class T> T plain_template(T x) { return x + 3; }
template<class T> inline T inline_template(T x) { return x + 4; }

int main()
{
  Box<int> box;
  box.value = 0;
  return box.in_class() + box.out_of_class() + box.inline_out_of_class() + plain_template(0) +
    inline_template(0) == 10 ? 0 : 1;
}
