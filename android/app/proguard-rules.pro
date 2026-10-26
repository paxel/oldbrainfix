# R8 still shrinks the release build; it only keeps the names. The crash
# report a user mails carries the plain stack trace, and with renamed
# classes and no mapping file next to it nobody could read it. The app is
# open source, so obfuscation hides nothing.
-dontobfuscate
-keepattributes SourceFile,LineNumberTable
