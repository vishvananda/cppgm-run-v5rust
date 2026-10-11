#include <vector>
#include <set>

struct Info {};

template <typename P>
void push(const Info & info, std::vector<P> & stack, std::set<P> & seen)
{
  (void)info;
  (void)stack;
  (void)seen;
}

template <typename P, typename Visitor>
bool walk(P root, Visitor visit)
{
  std::vector<P> stack;
  std::set<P> seen;
  push(*root, stack, seen);
  while(!stack.empty()) {
    P current = stack.back();
    stack.pop_back();
    if(!visit(current)) {
      return false;
    }
    push(*current, stack, seen);
  }
  return true;
}

bool derives_from(const Info & derived, const Info & base)
{
  bool found = false;
  walk(&derived, [&](const Info * current) {
    if(current == &base) {
      found = true;
      return false;
    }
    return true;
  });
  return found;
}
