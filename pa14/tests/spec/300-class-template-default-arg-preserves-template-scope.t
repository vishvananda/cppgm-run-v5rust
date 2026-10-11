// VALIDATION: compile-pass
// N3485 focus: 14.1 [temp.param], 14.5.1 [temp.class]

struct locale
{
  struct facet {};
  struct id {};
};

template<class T>
struct char_traits {};

template<class T>
struct allocator {};

template<class T, class Traits = char_traits<T>, class Alloc = allocator<T> >
struct basic_string {};

template<class CharT>
class collate : public locale::facet
{
public:
  typedef CharT char_type;
  typedef basic_string<char_type> string_type;
  static locale::id id;
};

template<class CharT>
locale::id collate<CharT>::id;

int main()
{
  return 0;
}
