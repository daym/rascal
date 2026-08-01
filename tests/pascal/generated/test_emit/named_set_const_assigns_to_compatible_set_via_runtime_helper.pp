unit cpupara;
interface
type
  tsmall = 0..15;
  tsmallset = set of tsmall;
  tbyteset = set of byte;
const
  rs_r0 = 0;
  rs_r3 = 3;
  rs_r12 = 12;
  rs_r15 = 15;
  volatile_intregisters = [rs_r0..rs_r3, rs_r12..rs_r15];
function take : tbyteset;
implementation
function take : tbyteset;
begin
  take := volatile_intregisters;
end;
end.
