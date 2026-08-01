unit pp;
interface
uses globals;
implementation
procedure run;
begin
  SetFPUExceptionMask([exInvalidOp, exDenormalized, exZeroDivide,
                       exOverflow, exUnderflow, exPrecision]);
end;
end.
