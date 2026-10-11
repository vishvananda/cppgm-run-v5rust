namespace N {
template<class T>
T min(T a, T b)
{
  return a < b ? a : b;
}
}

template<class T>
struct Box {
  static T f(T a, T b)
  {
    return (N::min)(a, b);
  }
};

int main()
{
  return Box<int>::f(3, 2);
}
