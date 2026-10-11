enum class MemoryOrder : int
{
  relaxed = 0,
  sequential = 5
};

MemoryOrder order = MemoryOrder::sequential;

int main()
{
  return order == MemoryOrder::sequential ? 0 : 1;
}
