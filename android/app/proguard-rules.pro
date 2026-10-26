# R8 still shrinks the release build; it only keeps the names. The crash
# report a user mails carries the plain stack trace, and with renamed
# classes and no mapping file next to it nobody could read it. The app is
# open source, so obfuscation hides nothing.
-dontobfuscate
-keepattributes SourceFile,LineNumberTable

# WorkManager starts at launch (androidx.startup) and makes its Room
# database by reflection. Room 2.6's own rule keeps the class only, and R8
# in full mode then drops the constructor: every release start died with
# NoSuchMethodException: androidx.work.impl.WorkDatabase_Impl.<init>.
-keep class * extends androidx.room.RoomDatabase { <init>(); }
