typedef decltype(sizeof(0)) size_t;

void * operator new(size_t, void * ptr);

template<class T>
T && declval();

struct Box {
  int value;
  Box() : value(7) {}
};

template<class T, class... Args,
         class = decltype(::new (declval<void *>()) T(declval<Args>()...))>
T * construct_at(T * ptr, Args &&... args)
{
  return ::new ((void *)ptr) T(static_cast<Args &&>(args)...);
}

int main()
{
  char storage[sizeof(Box)];
  Box * box = construct_at((Box *)storage);
  return box->value - 7;
}
