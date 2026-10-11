template<class... Primary>
struct owner {
  struct nested;
};

template<class... Bound>
struct owner<Bound...>::nested {
  static_assert(sizeof...(Bound) == 2, "");
};

int main()
{
  return sizeof(owner<int, long>::nested) == 1 ? 0 : 1;
}
