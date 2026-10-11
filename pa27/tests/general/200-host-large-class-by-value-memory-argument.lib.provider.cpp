struct Large {
  unsigned long words[17];
};

extern "C" int inspect_large_value(Large value, unsigned long tail)
{
  if(tail != 901) {
    return 1;
  }
  for(unsigned long i = 0; i < 17; ++i) {
    if(value.words[i] != 100 + i * 3) {
      return 2;
    }
  }
  return 0;
}

extern "C" int cppgm_inspect_large_value(Large value, unsigned long tail);

extern "C" int host_calls_cppgm_large_value()
{
  Large value = {};
  for(unsigned long i = 0; i < 17; ++i) {
    value.words[i] = 300 + i * 5;
  }
  return cppgm_inspect_large_value(value, 777);
}
