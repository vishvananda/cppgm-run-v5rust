#include <random>

using Engine = std::mt19937;

int random_mersenne_anchor()
{
  Engine engine;
  return static_cast<int>(engine());
}
