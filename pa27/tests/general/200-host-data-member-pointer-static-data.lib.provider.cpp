#include "200-host-data-member-pointer-static-data.helper.h"

namespace member_data {
Member host_null;
Member host_b = &S::b;
Entry host_unset;
Entry host_entries[4] = { { &S::a, 1 }, { 0, 2 }, { &S::c, 3 } };
Table host_table = { 2, { { &S::b, 4 } } };

static bool unset(const Entry& entry) { return !entry.member && entry.tag == 0; }

int host_check_student()
{
  int status = 0;
  if (student_null || student_c != &S::c) status |= 1;
  if (!unset(student_unset)) status |= 2;
  if (student_entries[0].member != &S::a || student_entries[1].member ||
      student_entries[2].member != &S::c || !unset(student_entries[3])) status |= 4;
  if (student_table.count != 2 || student_table.entries[0].member != &S::b ||
      !unset(student_table.entries[1]) || !unset(student_table.entries[2]) ||
      student_table.extra) status |= 8;
  return status;
}
}
