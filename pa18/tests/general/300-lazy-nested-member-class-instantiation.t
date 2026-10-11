// Instantiating a class-template member class declares its nested member
// classes, but must not instantiate their definitions unless they are used.

template<class T>
struct outer {
  struct holder {
    struct unused {
      enum { value = sizeof(typename T::missing) };
    };

    explicit holder(long v)
      : value((int)v)
    {
    }

    holder(T *)
      : value(unused::value)
    {
    }

    int value;
  };
};

typedef outer<int>::holder holder;

void take(const holder &)
{
}

int main()
{
  holder h(5L);
  take(h);
  return h.value - 5;
}
