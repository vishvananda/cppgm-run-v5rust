#include <utility>

struct MoveOnly {
  MoveOnly();
  MoveOnly(const MoveOnly &) = delete;
  MoveOnly(MoveOnly &&);
};

void consume(MoveOnly &&);

void probe(MoveOnly &value)
{
  MoveOnly moved(std::move(value));
  consume(std::move(moved));
}
