# pull an ISO timestamp out of a line of text
# example: `cat LOGFILE | ./extract-timestamp.sh`

perl -pE 's/.*(\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[1-2]\d|3[0-1])T(?:[0-1]\d|2[0-3]):[0-5]\d:[0-5]\d(?:\.\d+|)(?:Z|(?:\+|\-)(?:\d{2}):?(?:\d{2}))).*/\1/g'
