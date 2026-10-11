struct X {};

template<class T>
struct pointer_arg {
  typedef void * type;
};

template<class T>
struct int_arg {
  typedef int type;
};

template<class T>
int select(void (*)(T), typename pointer_arg<T>::type);

template<class T>
char select(void (*)(T), typename int_arg<T>::type);

static_assert(sizeof(select((void (*)(X))0, 1)) == sizeof(char), "");

int main()
{
  return sizeof(select((void (*)(X))0, 1)) == sizeof(char) ? 0 : 1;
}
