// N3485 4.9/1 + 3.6.2/2 (PA29 __int128): (unsigned __int128)1e30 is 1e30
// exactly (needs 100 bits); the constant initializer must not saturate/zero
// beyond 64 bits
