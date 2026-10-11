namespace functions {

template<class T>
T item(T value);

}

namespace types {

template<class T>
struct item {
};

}

struct result {
};

result declared(types::item<int>(value));

int main()
{
  return 0;
}
