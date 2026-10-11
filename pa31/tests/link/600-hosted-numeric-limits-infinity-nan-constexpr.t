// N3485 18.3.2.3:
// numeric_limits<double>::infinity()/quiet_NaN()/signaling_NaN() are
// constexpr; hosted <limits> forwards to
// __builtin_inf/__builtin_nan/__builtin_nans, which must fold
