template<class... Args>
__attribute__((noinline)) int handle(void* block, unsigned long size, Args... args)
{
  return block != 0 && size != 0 ? 1 : 0;
}
