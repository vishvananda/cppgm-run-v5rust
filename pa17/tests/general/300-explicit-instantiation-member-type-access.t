// An explicit instantiation names its member after the class, and the member's
// declared type is named with that member's access just as its declarator is.
// libc++ declares
//   extern template __time_get_storage<char>::string_type
//   __time_get_storage<char>::__analyze(char, const ctype<char>&);
// where string_type is protected in that class, so building the return type
// outside the class judges that access from no class at all.

template<class T>
struct storage
{
protected:
  typedef T *pointer_type;
  pointer_type analyze(char);
};

template<class T>
typename storage<T>::pointer_type storage<T>::analyze(char)
{
  return 0;
}

// The declaration suppresses the implicit instantiation; the definition
// supplies it.  Both name the protected member as the return type.
extern template storage<char>::pointer_type storage<char>::analyze(char);
template storage<char>::pointer_type storage<char>::analyze(char);

int main()
{
  storage<char> made;
  (void)made;
  return 0;
}
