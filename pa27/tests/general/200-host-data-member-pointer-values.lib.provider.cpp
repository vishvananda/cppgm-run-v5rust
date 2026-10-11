#include "200-host-data-member-pointer-values.helper.h"

namespace member_values {
int S::* host_member(int which)
{
  if (which == 0) return &S::a;
  if (which == 1) return &S::b;
  if (which == 2) return &S::c;
  return 0;
}

bool host_is_null(int S::* pointer) { return !pointer; }
int host_read(const S& object, int S::* pointer) { return object.*pointer; }
int D::* host_widen(int B2::* pointer) { return pointer; }

int host_check_student()
{
  int status = 0;
  S object = { 10, 20, 30 };
  if (student_member(0) != &S::a || student_member(2) != &S::c ||
      object.*student_member(1) != 20) status |= 1;
  if (student_member(3) != 0 || !student_is_null(0) ||
      student_is_null(&S::a)) status |= 2;
  if (student_read(object, &S::a) != 10 || student_read(object, &S::c) != 30)
    status |= 4;
  D derived;
  derived.y = 7;
  derived.z = 9;
  if (derived.*student_widen(&B2::y) != 7 || derived.*student_widen(&B2::z) != 9 ||
      student_widen(0) != 0) status |= 8;
  return status;
}
}
