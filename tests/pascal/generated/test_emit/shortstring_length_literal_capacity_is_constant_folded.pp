unit u;
interface
type tsection = (sec_code, sec_data);
const secnames : array[tsection] of string[length('__DATA, __datacoal_nt,coalesced')] = ('__TEXT', '__DATA, __datacoal_nt,coalesced');
implementation
end.
