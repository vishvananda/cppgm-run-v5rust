// expect reject
// Alignment operands are typed before integral evaluation, including address operands.
int object;
alignas(&object) int aligned;
