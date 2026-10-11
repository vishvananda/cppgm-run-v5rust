// A precollected operator candidate set must not repeat ADL and admit later
// declarations while analyzing an inline function body.
namespace logic {

class tribool;
struct marker {};
typedef bool (*keyword)(tribool, marker);

bool indeterminate(tribool, marker = marker());

class tribool
{
public:
  tribool(bool initial)
    : value(initial ? yes : no)
  {
  }

  tribool(keyword)
    : value(maybe)
  {
  }

  enum value_t { no, yes, maybe } value;
};

template<class T>
marker operator==(marker, T);

inline bool indeterminate(tribool value, marker)
{
  return value.value == tribool::maybe;
}

tribool operator==(tribool lhs, tribool) { return lhs; }
tribool operator==(tribool lhs, bool) { return lhs; }
tribool operator==(bool, tribool rhs) { return rhs; }

}

int main()
{
  logic::tribool value(false);
  return logic::indeterminate(value) ? 1 : 0;
}
