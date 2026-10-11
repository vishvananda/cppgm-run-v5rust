// N3485 15.1p8, 15.3: a rethrown long, caught after a destructor run during
// the unwinding caught a nested int exception, is caught by catch (long), not
// by catch (char)
