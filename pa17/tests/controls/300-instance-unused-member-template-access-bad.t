// expect reject
// N3485 14.7.1p1: the member-template declaration is instantiated with
// its enclosing class even though no call or body is requested.
template<class T> class hidden { typedef int result; };
template<class T> struct owner
{
  template<class U> typename hidden<T>::result unused(U);
};

int main() { owner<int> object; return 0; }
