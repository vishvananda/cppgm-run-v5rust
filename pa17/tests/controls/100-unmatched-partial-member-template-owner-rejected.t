// expect reject
// An enclosing template head cannot invent a partial owner for a member template.
template<class T>
struct Owner
{
  template<class U> int value();
};

template<class T>
template<class U>
int Owner<T *>::value()
{
  return sizeof(U);
}
