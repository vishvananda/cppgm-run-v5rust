struct SystemMoveData
{
  long * ptr;

  SystemMoveData() : ptr(0) {}
  SystemMoveData(const SystemMoveData &) = default;
  SystemMoveData(SystemMoveData && other) : SystemMoveData(other)
  {
    other.reset();
  }

  void reset()
  {
    ptr = 0;
  }
};
