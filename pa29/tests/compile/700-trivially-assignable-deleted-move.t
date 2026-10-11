struct move_track {
  move_track(move_track &&);
};
static_assert(!__is_trivially_assignable(move_track &, move_track &&), "");
int main() {}
