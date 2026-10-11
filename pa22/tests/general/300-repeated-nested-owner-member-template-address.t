// VALIDATION: compile-pass
// Repeated instantiations of one enclosing member-template body must retain
// the distinct nested member-template specialization named by each address.

template<class T>
struct owner
{
  struct nested
  {
    typedef void (nested::*pointer)();
    pointer value;

  private:
    template<class U>
    void function();

    friend struct owner;
  };

  template<class U>
  void select()
  {
    nested value;
    value.value = &nested::template function<U>;
    (value.*value.value)();
  }
};

template<class T>
template<class U>
void owner<T>::nested::function()
{
}

int main()
{
  owner<int> value;
  value.select<char>();
  value.select<short>();
  value.select<long>();
  return 0;
}
