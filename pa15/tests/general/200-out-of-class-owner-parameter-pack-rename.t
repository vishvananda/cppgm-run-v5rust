template<class... Primary>
struct owner {
  owner(Primary const&...);
};

template<class... Bound>
owner<Bound...>::owner(Bound const&...)
{
}

int main()
{
  return 0;
}
