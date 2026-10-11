#ifndef CPPGM_244_HOST_EH_UNWIND_THROUGH_PLAIN_FRAME_HELPER_H
#define CPPGM_244_HOST_EH_UNWIND_THROUGH_PLAIN_FRAME_HELPER_H

struct Marker
{
  explicit Marker(int v) : value(v) {}
  int value;
};

int keep_live(int a, int b, int c, int d);
void plain_frame(int value);

#endif
