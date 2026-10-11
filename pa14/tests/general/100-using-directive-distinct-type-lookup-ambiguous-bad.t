namespace left {
struct token {};
}

namespace right {
struct token {};
}

template<class T>
struct box {
  T item;
};

int main()
{
  using namespace left;
  using namespace right;
  box<token> x;
  return sizeof(x);
}
