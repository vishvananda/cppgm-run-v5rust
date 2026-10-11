inline void pause_once()
{
  __asm__ __volatile__("rep; nop" : : : "memory");
}

int main()
{
  pause_once();
  return 0;
}
