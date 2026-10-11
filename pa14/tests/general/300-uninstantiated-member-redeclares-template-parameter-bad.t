// N3485 focus: 14.6.1 [temp.local] a template parameter shall not be
// redeclared within its scope, including an uninstantiated member body.

template<class T>
struct box
{
  void touch();
};

template<class T>
void box<T>::touch()
{
  using T = int;
}
