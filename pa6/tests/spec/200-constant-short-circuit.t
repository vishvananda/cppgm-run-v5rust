static_assert(1 || (1 / 0), "short circuit");
static_assert(!(0 && (1 / 0)), "short circuit");
