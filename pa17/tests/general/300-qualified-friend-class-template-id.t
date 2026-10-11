// VALIDATION: compile-pass

namespace detail {

template<class T>
class interface_archive
{
};

}

template<class T>
class archive
{
  friend class detail::interface_archive<T>;
};

int main()
{
  return 0;
}
