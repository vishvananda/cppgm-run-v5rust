// N3485 13.3.1.2p3, 13.3.3p1: a function template specialization does not
// suppress the builtin enum comparison; its tied conversions lose to the
// builtin. The unused dependent body must remain uninstantiated.
// expect exit 0
enum E { first, second };
template<class T> bool operator==(const T&, const T&)
{
  return T::missing;
}
int main()
{
  return first == second;
}
