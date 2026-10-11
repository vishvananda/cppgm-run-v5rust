struct yes { char c; };
struct no { char c[2]; };

template<class T>
T&& declval();

template<class T>
yes can_assign(int, decltype(declval<T&>() = declval<T&&>())* = 0);

template<class T>
no can_assign(...);

template<class T>
struct owner {
  int run()
  {
    auto task = [this]() {};
    return sizeof(can_assign<decltype(task)>(0)) == sizeof(no) ? 0 : 1;
  }
};

int main()
{
  owner<int> o;
  return o.run();
}
